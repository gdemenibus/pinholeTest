#!/usr/bin/env fish
set files (ls ./resources/cycle/)
set perfect ./resources/cycle/Perfect.png
for file in $files
	set score (dssim ./resources/cycle/Perfect$file ./resources/cycle/$file)
	echo $score
end
