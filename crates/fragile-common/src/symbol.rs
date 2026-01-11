use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::sync::RwLock;

/// An interned string identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(u32);

impl Symbol {
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// Thread-safe string interner for symbols.
#[derive(Debug, Default)]
pub struct SymbolInterner {
    map: RwLock<FxHashMap<SmolStr, Symbol>>,
    strings: RwLock<Vec<SmolStr>>,
}

impl SymbolInterner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&self, s: &str) -> Symbol {
        // Fast path: check if already interned
        {
            let map = self.map.read().unwrap();
            if let Some(&sym) = map.get(s) {
                return sym;
            }
        }

        // Slow path: insert new symbol
        let mut map = self.map.write().unwrap();
        let mut strings = self.strings.write().unwrap();

        // Double-check after acquiring write lock
        if let Some(&sym) = map.get(s) {
            return sym;
        }

        let sym = Symbol(strings.len() as u32);
        let smol = SmolStr::new(s);
        strings.push(smol.clone());
        map.insert(smol, sym);
        sym
    }

    pub fn resolve(&self, sym: Symbol) -> SmolStr {
        let strings = self.strings.read().unwrap();
        strings[sym.0 as usize].clone()
    }
}
