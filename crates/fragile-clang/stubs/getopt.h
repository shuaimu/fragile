// Minimal getopt.h stub for fragile parsing
#ifndef _GETOPT_H
#define _GETOPT_H

#ifdef __cplusplus
extern "C" {
#endif

// Option value for missing argument
#define no_argument        0
#define required_argument  1
#define optional_argument  2

// Long option structure
struct option {
    const char *name;    // Name of the option
    int         has_arg; // no_argument, required_argument, optional_argument
    int        *flag;    // If non-NULL, set *flag to val when option found
    int         val;     // Value to return (or store in *flag)
};

// Global variables set by getopt
extern char *optarg;  // Argument for the current option
extern int   optind;  // Index of next element in argv
extern int   opterr;  // If non-zero, print error messages
extern int   optopt;  // Character that caused error

// getopt functions
int getopt(int argc, char * const argv[], const char *optstring);
int getopt_long(int argc, char * const argv[], const char *optstring,
                const struct option *longopts, int *longindex);
int getopt_long_only(int argc, char * const argv[], const char *optstring,
                     const struct option *longopts, int *longindex);

#ifdef __cplusplus
}
#endif

#endif // _GETOPT_H
