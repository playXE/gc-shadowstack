# gc-shadowstack

Implementation of shadow stack to implement object rooting in GC libraries.

# Description

Unlike many GC algorithms which rely on a cooperative code generator to compile stack maps (no support in rustc), this algorithm carefully maintains a linked list of stack roots [Henderson2002]. This so-called “shadow stack” mirrors the machine stack. Maintaining this data structure is slower than using a stack map compiled into the executable as constant data, but has a significant portability advantage because it requires no special support from the target code generator, and does not require tricky platform-specific code to crawl the machine stack nor it requires heap allocation and reference counting for maintaining rooted object list.