# vimrs
A vim clone written in Rust by mostly following a Medium tutorial by Kofi Otuo


Goals:

Markdown preview
Text formatting/coloring with Colorize crate
Getting it pretty close to Vim
Line numbers and syntax highlighting


I think theres a bug when trying to use colorize and crossterm colors. 
Currently the push_str method has been modified to support coloring the tilde and line numbers.