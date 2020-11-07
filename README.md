# Alligator

`Alligator` is a test tool for program to manipulate memory directly.

Rust usually follows the rule of RAII (Resource Acquisition is Initialization;)
resources should be released on the drop, however, it is sometimes difficult to design low
level code like that.

Container object, for example, sometimes allocates heap memory and build elements there.
Then, the programmer could have to drop the elements and deallocate the heap manully; otherwise
some trouble like memory leak could be occurred.

`Alligator` helps to test such program.

[lgpl-badge]: https://img.shields.io/badge/license-lgpl-blue.svg
[apache2-badge]: https://img.shields.io/badge/license-apache2-blue.svg
[mit-badge]: https://img.shields.io/badge/license-mit-blue.svg
[bsd2-badge]: https://img.shields.io/badge/license-bsd2-blue.svg
