#!/bin/sh

# download blog corpus
wget http://www.cs.biu.ac.il/~koppel/blogs/blogs.zip

# unzip blog corpus
unzip blogs.zip

# concatenate to one file remove any non-ascii character
# the data contains many different encodings. 
# this is the easiest way to ensure that there is no invalid unicode
cat blogs/* | tr -c -d '\000-\177' > blog-corpus-ascii
