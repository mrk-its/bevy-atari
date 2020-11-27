#!/usr/bin/perl
use strict;

if (@ARGV == 0) {
	print "Usage: perl act2html.pl INPUT.act... > OUTPUT.html\n";
	exit;
}
my @files = glob "@ARGV";
@files > 0 or die "act2html.pl: no input file\n";

my @palettes;
for (@files) {
	my $pal;
	open IN, $_ and read IN, $pal, 769 and close IN or die "act2html.pl: $_: $!\n";
	die "act2html.pl: $_ is too short\n" if length($pal) < 768;
	die "act2html.pl: $_ is too long\n" if length($pal) > 768;
	push @palettes, $pal;
}
print "pub const PALETTE: &[[u8; 3]] = &[\n";
for my $hue (0 .. 15) {
	for my $i (0 .. $#files) {
		for (my $color = $hue * 16; ; ) {
			my $rgb = substr($palettes[$i], $color * 3, 3);
			my $colspan = 1;
			while (($color & 0xf) < 0xf && $rgb eq substr($palettes[$i], $color * 3 + 3, 3)) {
				$colspan++;
				$color++;
			}
			my $r = unpack('H2', substr($rgb, 0, 1));
			my $g = unpack('H2', substr($rgb, 1, 2));
			my $b = unpack('H2', substr($rgb, 2, 3));
			print "    [0x$r, 0x$g, 0x$b],\n";
			++$color & 0xf or last;
		}
	}
}
print "];\n";