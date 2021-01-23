# gharial

`Gharial` is a test tool for program to manipulate memory directly.

Rust usually follows the rule of RAII (Resource Acquisition is Initialization;)
resources should be released on the drop, however, it is sometimes difficult to design low
level code like that.

Container object, for example, sometimes allocates heap memory and build elements there.
Then, the programmer could have to drop the elements and deallocate the heap manully; otherwise
some trouble like memory leak could be occurred.

`Gharial` helps to test such program.

License: LGPL-3.0-or-later OR Apache-2.0 OR BSD-2-Clause OR MIT
