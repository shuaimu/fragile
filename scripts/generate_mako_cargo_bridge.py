#!/usr/bin/env python3
"""
Generate a Cargo bridge crate for a Mako CMake target.

The generated crate uses build.rs to:
1) replay compile_commands entries for the target via fragilec -c
2) replay the CMake link line for the target via fragilec

This is a scaffolding bridge for incremental migration, not a full Cargo-native
rewrite of all CMake logic.
"""

from __future__ import annotations

import argparse
import json
import re
import shlex
import textwrap
from pathlib import Path
from typing import Any


def json_string(value: str) -> str:
    return json.dumps(value)


def load_compile_commands(path: Path) -> list[dict[str, Any]]:
    raw = json.loads(path.read_text())
    if not isinstance(raw, list):
        raise ValueError(f"expected list in {path}")
    return raw


def command_tokens(entry: dict[str, Any]) -> list[str]:
    arguments = entry.get("arguments")
    if isinstance(arguments, list) and arguments:
        return [str(x) for x in arguments]
    command = entry.get("command")
    if not isinstance(command, str) or not command.strip():
        raise ValueError(f"compile_commands entry missing command/arguments: {entry}")
    return shlex.split(command)


def looks_like_same_path(token: str, source_path: Path) -> bool:
    if token.startswith("-"):
        return False
    try:
        return Path(token).resolve() == source_path.resolve()
    except OSError:
        return False


def sanitize_compile_flags(tokens: list[str], source: Path) -> list[str]:
    # Drop compile-driver output/dependency flags and explicit source token.
    out: list[str] = []
    i = 1  # skip compiler path at argv[0]
    while i < len(tokens):
        tok = tokens[i]
        if tok in ("-c", "--compile", "-fPIC"):
            i += 1
            continue
        if tok in ("-o", "--output", "-MF", "-MT", "-MQ", "-MJ"):
            i += 2
            continue
        if tok in ("-MD", "-MMD", "-MP"):
            i += 1
            continue
        if tok.startswith("-o") and tok != "-o":
            i += 1
            continue
        if looks_like_same_path(tok, source):
            i += 1
            continue
        out.append(tok)
        i += 1
    return out


def sanitize_rel_for_filename(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "_", value)


def resolve_output_path(build_dir: Path, output_token: str) -> Path:
    output_path = Path(output_token)
    if output_path.is_absolute():
        return output_path.resolve()
    return (build_dir / output_path).resolve()


def collect_target_compile_units(
    compile_commands: list[dict[str, Any]], build_dir: Path, target: str
) -> list[dict[str, Any]]:
    target_prefix = f"CMakeFiles/{target}.dir/"
    selected_by_output: dict[str, dict[str, Any]] = {}

    for entry in compile_commands:
        output = entry.get("output")
        source = entry.get("file")
        if not isinstance(output, str) or not isinstance(source, str):
            continue
        if target_prefix not in output or not output.endswith(".o"):
            continue
        selected_by_output[output] = entry

    if not selected_by_output:
        raise ValueError(
            f"no compile_commands entries matched target '{target}' with prefix '{target_prefix}'"
        )

    units: list[dict[str, Any]] = []
    for idx, output in enumerate(sorted(selected_by_output.keys())):
        entry = selected_by_output[output]
        source = Path(str(entry["file"])).resolve()
        tokens = command_tokens(entry)
        flags = sanitize_compile_flags(tokens, source)
        obj_rel = f"obj/{idx:03d}_{sanitize_rel_for_filename(output)}"
        units.append(
            {
                "source": str(source),
                "original_output": output,
                "original_output_abs": str(resolve_output_path(build_dir, output)),
                "object_rel": obj_rel,
                "flags": flags,
            }
        )
    return units


def parse_link_tokens(link_txt_path: Path) -> list[str]:
    raw = link_txt_path.read_text().strip()
    if not raw:
        raise ValueError(f"empty link.txt: {link_txt_path}")
    return shlex.split(raw)


def extract_link_spec(
    build_dir: Path, target: str, link_tokens: list[str], compile_units: list[dict[str, Any]]
) -> tuple[str, list[str]]:
    object_tokens: set[str] = set()
    object_abs: set[str] = set()
    for unit in compile_units:
        rel = str(unit["original_output"])
        abs_path = str(unit["original_output_abs"])
        object_tokens.add(rel)
        object_abs.add(abs_path)

    output_name = target
    link_args: list[str] = []

    i = 1  # skip link driver argv[0]
    while i < len(link_tokens):
        tok = link_tokens[i]

        if tok == "-o" and i + 1 < len(link_tokens):
            output_name = Path(link_tokens[i + 1]).name
            i += 2
            continue
        if tok.startswith("-o") and tok != "-o":
            output_name = Path(tok[2:]).name
            i += 1
            continue

        # The bridge re-emits dependency discovery via Cargo itself.
        if tok.startswith("-Wl,--dependency-file="):
            i += 1
            continue

        if tok in object_tokens:
            i += 1
            continue
        if tok.endswith(".o"):
            tok_abs = str(resolve_output_path(build_dir, tok))
            if tok_abs in object_abs:
                i += 1
                continue

        link_args.append(tok)
        i += 1

    return output_name, link_args


def build_rs_text(
    default_build_dir: str,
    output_name: str,
    target: str,
    compile_units: list[dict[str, Any]],
    link_args: list[str],
) -> str:
    compile_lines: list[str] = []
    for unit in compile_units:
        flags = ", ".join(json_string(flag) for flag in unit["flags"])
        compile_lines.append(
            f"    CompileUnit {{ source: {json_string(unit['source'])}, object_rel: {json_string(unit['object_rel'])}, flags: &[{flags}] }},"
        )

    link_lines = "\n".join(f"    {json_string(arg)}," for arg in link_args)

    return textwrap.dedent(
        f"""\
        use std::env;
        use std::fs;
        use std::path::PathBuf;
        use std::process::Command;

        const DEFAULT_MAKO_BUILD_DIR: &str = {json_string(default_build_dir)};
        const TARGET_NAME: &str = {json_string(target)};
        const OUTPUT_NAME: &str = {json_string(output_name)};

        struct CompileUnit {{
            source: &'static str,
            object_rel: &'static str,
            flags: &'static [&'static str],
        }}

        static COMPILE_UNITS: &[CompileUnit] = &[
        {chr(10).join(compile_lines)}
        ];

        static LINK_ARGS: &[&str] = &[
        {link_lines}
        ];

        fn run_checked(mut cmd: Command, stage: &str) -> Result<(), String> {{
            let output = cmd
                .output()
                .map_err(|e| format!("{{stage}}: failed to spawn command: {{e}}"))?;
            if !output.status.success() {{
                return Err(format!(
                    "{{stage}}: command failed\\nstatus: {{}}\\nstdout:\\n{{}}\\nstderr:\\n{{}}",
                    output.status,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ));
            }}
            Ok(())
        }}

        fn expand_wl_forwarded_args(arg: &str) -> Vec<String> {{
            if let Some(rest) = arg.strip_prefix("-Wl,") {{
                return rest
                    .split(',')
                    .filter(|part| !part.is_empty())
                    .map(|part| part.to_string())
                    .collect();
            }}
            vec![arg.to_string()]
        }}

        fn normalize_link_args_for_rust_lld(args: &[&str]) -> Vec<String> {{
            let mut normalized: Vec<String> = Vec::new();
            for arg in args {{
                if matches!(*arg, "-fPIC" | "-DNDEBUG") {{
                    continue;
                }}
                if *arg == "-pthread" {{
                    normalized.push("-lpthread".to_string());
                    continue;
                }}
                if arg.starts_with("-Wl,") {{
                    for token in expand_wl_forwarded_args(arg) {{
                        if token.starts_with("--dependency-file=") {{
                            continue;
                        }}
                        normalized.push(token);
                    }}
                    continue;
                }}
                normalized.push((*arg).to_string());
            }}
            normalized
        }}

        fn default_native_search_dirs() -> Vec<String> {{
            let mut dirs: Vec<String> = vec![
                "/usr/lib/x86_64-linux-gnu".to_string(),
                "/lib/x86_64-linux-gnu".to_string(),
                "/usr/lib/gcc/x86_64-linux-gnu/14".to_string(),
            ];

            if let Ok(extra) = env::var("RUST_LLD_NATIVE_DIRS") {{
                for dir in extra.split(':').map(|v| v.trim()).filter(|v| !v.is_empty()) {{
                    if !dirs.iter().any(|existing| existing == dir) {{
                        dirs.push(dir.to_string());
                    }}
                }}
            }}

            dirs
        }}

        fn main() {{
            println!("cargo:rerun-if-env-changed=FRAGILEC_BIN");
            println!("cargo:rerun-if-env-changed=FRAGILEC_MODE");
            println!("cargo:rerun-if-env-changed=RUSTC_BIN");
            println!("cargo:rerun-if-env-changed=RUST_LLD_NATIVE_DIRS");
            println!("cargo:rerun-if-env-changed=MAKO_BUILD_DIR");
            println!("cargo:rerun-if-changed=build.rs");
            println!("cargo:rerun-if-changed=README.md");

            let build_dir = PathBuf::from(
                env::var("MAKO_BUILD_DIR").unwrap_or_else(|_| DEFAULT_MAKO_BUILD_DIR.to_string()),
            );
            let fragilec = env::var("FRAGILEC_BIN").unwrap_or_else(|_| "fragilec".to_string());
            let rustc = env::var("RUSTC_BIN").unwrap_or_else(|_| "rustc".to_string());
            let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set"));
            let obj_root = out_dir.join("mako_obj");
            let bin_root = out_dir.join("mako_bin");
            let linked_bin = bin_root.join(OUTPUT_NAME);
            let link_src = out_dir.join("mako_bridge_link.rs");

            if let Err(err) = fs::create_dir_all(&obj_root) {{
                panic!("failed to create object root {{}}: {{err}}", obj_root.display());
            }}
            if let Err(err) = fs::create_dir_all(&bin_root) {{
                panic!("failed to create binary root {{}}: {{err}}", bin_root.display());
            }}
            if let Err(err) = fs::write(&link_src, "#![no_main]\\n") {{
                panic!("failed to write bridge link source {{}}: {{err}}", link_src.display());
            }}

            for unit in COMPILE_UNITS {{
                println!("cargo:rerun-if-changed={{}}", unit.source);
                let out_obj = obj_root.join(unit.object_rel);
                if let Some(parent) = out_obj.parent() {{
                    if let Err(err) = fs::create_dir_all(parent) {{
                        panic!(
                            "failed to create object parent {{}}: {{err}}",
                            parent.display()
                        );
                    }}
                }}

                let mut cmd = Command::new(&fragilec);
                cmd.current_dir(&build_dir);
                if env::var_os("FRAGILEC_MODE").is_none() {{
                    cmd.env("FRAGILEC_MODE", "strict");
                }}
                cmd.args(unit.flags);
                cmd.arg("-c");
                cmd.arg(unit.source);
                cmd.arg("-o");
                cmd.arg(&out_obj);
                if let Err(err) = run_checked(cmd, "compile") {{
                    panic!(
                        "mako cargo bridge compile failed for target '{{}}' unit '{{}}':\\n{{}}",
                        TARGET_NAME, unit.source, err
                    );
                }}
            }}

            let mut link_cmd = Command::new(&rustc);
            link_cmd.current_dir(&build_dir);
            link_cmd.arg("--edition");
            link_cmd.arg("2021");
            link_cmd.arg("-C");
            link_cmd.arg("linker=rust-lld");
            link_cmd.arg("-C");
            link_cmd.arg("panic=abort");

            for native_dir in default_native_search_dirs() {{
                link_cmd.arg("-L");
                link_cmd.arg(format!("native={{native_dir}}"));
            }}

            link_cmd.arg(&link_src);
            link_cmd.arg("-o");
            link_cmd.arg(&linked_bin);

            for unit in COMPILE_UNITS {{
                link_cmd.arg("-C");
                link_cmd.arg(format!(
                    "link-arg={{}}",
                    obj_root.join(unit.object_rel).display()
                ));
            }}

            for arg in normalize_link_args_for_rust_lld(LINK_ARGS) {{
                link_cmd.arg("-C");
                link_cmd.arg(format!("link-arg={{arg}}"));
            }}

            // Keep core C/POSIX runtime link flags explicit for raw rust-lld usage.
            for c_lib in ["-lgcc_s", "-lutil", "-lrt", "-lpthread", "-lm", "-ldl", "-lc"] {{
                link_cmd.arg("-C");
                link_cmd.arg(format!("link-arg={{c_lib}}"));
            }}

            if let Err(err) = run_checked(link_cmd, "link") {{
                panic!(
                    "mako cargo bridge rust-lld link failed for target '{{}}':\\n{{}}",
                    TARGET_NAME, err
                );
            }}

            let manifest_dir = PathBuf::from(
                env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set"),
            );
            let dist_dir = manifest_dir.join("dist");
            if let Err(err) = fs::create_dir_all(&dist_dir) {{
                panic!("failed to create dist dir {{}}: {{err}}", dist_dir.display());
            }}
            let dist_bin = dist_dir.join(OUTPUT_NAME);
            if let Err(err) = fs::copy(&linked_bin, &dist_bin) {{
                panic!(
                    "failed to copy linked binary from {{}} to {{}}: {{err}}",
                    linked_bin.display(),
                    dist_bin.display()
                );
            }}
            println!(
                "cargo:warning=mako cargo bridge built '{{}}' at {{}}",
                OUTPUT_NAME,
                dist_bin.display()
            );
        }}
        """
    )


def cargo_toml_text(package_name: str) -> str:
    return textwrap.dedent(
        f"""\
        [package]
        name = {json_string(package_name)}
        version = "0.1.0"
        edition = "2021"
        publish = false
        build = "build.rs"

        [dependencies]
        """
    )


def readme_text(target: str, build_dir: Path) -> str:
    return textwrap.dedent(
        f"""\
        # Mako Cargo Bridge ({target})

        This crate is generated from CMake artifacts and bridges one Mako target into
        Cargo orchestration.

        Source inputs:
        - `compile_commands.json` (target compile units)
        - `CMakeFiles/{target}.dir/link.txt` (target link line)

        ## Build

        ```bash
        # From this crate directory
        FRAGILEC_BIN=/home/shuai/workspace/fragile/target/release/fragilec \\
        RUSTC_BIN=rustc \\
        MAKO_BUILD_DIR={build_dir} \\
        cargo build
        ```

        Notes:
        - Final link is driven by `rustc -C linker=rust-lld`.
        - Default native search dirs for rust-lld:
          `/usr/lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu:/usr/lib/gcc/x86_64-linux-gnu/14`
        - Override/add with `RUST_LLD_NATIVE_DIRS=/path1:/path2`.

        The resulting linked target is copied to:
        - `dist/{target}`
        """
    )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate a Cargo bridge crate from Mako CMake compile/link artifacts."
    )
    parser.add_argument(
        "--build-dir",
        required=True,
        type=Path,
        help="Mako CMake build directory containing compile_commands.json",
    )
    parser.add_argument(
        "--target",
        default="test_rpc",
        help="CMake target name to bridge (default: test_rpc)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=None,
        help="Output Cargo crate directory (default: <mako-root>/cargo-mako-<target>)",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="Overwrite output directory if it already exists",
    )
    args = parser.parse_args()

    build_dir = args.build_dir.resolve()
    compile_commands_path = build_dir / "compile_commands.json"
    link_txt_path = build_dir / "CMakeFiles" / f"{args.target}.dir" / "link.txt"

    if not compile_commands_path.is_file():
        raise SystemExit(f"missing compile_commands.json: {compile_commands_path}")
    if not link_txt_path.is_file():
        raise SystemExit(f"missing target link.txt: {link_txt_path}")

    if args.output_dir is None:
        output_dir = build_dir.parent / f"cargo-mako-{args.target}"
    else:
        output_dir = args.output_dir.resolve()

    if output_dir.exists() and not args.force:
        raise SystemExit(
            f"output directory already exists: {output_dir} (use --force to overwrite)"
        )

    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "src").mkdir(parents=True, exist_ok=True)

    compile_commands = load_compile_commands(compile_commands_path)
    compile_units = collect_target_compile_units(compile_commands, build_dir, args.target)
    link_tokens = parse_link_tokens(link_txt_path)
    output_name, link_args = extract_link_spec(build_dir, args.target, link_tokens, compile_units)

    package_name = f"mako-{args.target.replace('_', '-')}-bridge"

    spec = {
        "build_dir": str(build_dir),
        "target": args.target,
        "output_name": output_name,
        "compile_units": compile_units,
        "link_args": link_args,
    }
    (output_dir / "mako_bridge_spec.json").write_text(json.dumps(spec, indent=2) + "\n")
    (output_dir / "build.rs").write_text(
        build_rs_text(str(build_dir), output_name, args.target, compile_units, link_args)
    )
    (output_dir / "Cargo.toml").write_text(cargo_toml_text(package_name))
    (output_dir / "src" / "main.rs").write_text(
        textwrap.dedent(
            f"""\
            fn main() {{
                println!(
                    "Mako Cargo bridge crate for target '{args.target}'. See dist/{output_name} after build."
                );
            }}
            """
        )
    )
    (output_dir / "README.md").write_text(readme_text(args.target, build_dir))
    (output_dir / ".gitignore").write_text("target/\ndist/\n")

    print(f"generated Cargo bridge crate: {output_dir}")
    print(f"target: {args.target}")
    print(f"compile units: {len(compile_units)}")
    print(f"link args: {len(link_args)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
