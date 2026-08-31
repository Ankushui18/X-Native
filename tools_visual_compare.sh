#!/bin/bash
# Visual export regression: rasterize SVG + PDF exports and RMSE-compare
# against the GPU canvas render. Usage: ./tools_visual_compare.sh [threshold]
cd "$(dirname "$0")/export_fixtures" || exit 1
TH=${1:-0.30}   # RMSE threshold (0..1 scale)
fail=0
printf "%-12s %-10s %-10s\n" FIXTURE SVG_RMSE PDF_RMSE
for c in canvas_*.png; do
  name=${c#canvas_}; name=${name%.png}
  rsvg-convert -b white -w 400 -h 300 "svg_${name}.svg" -o "rast_svg_${name}.png" 2>/dev/null
  gs -q -dNOPAUSE -dBATCH -sDEVICE=png16m -dGraphicsAlphaBits=4 -dTextAlphaBits=4 -r72 -g400x300 -sOutputFile="rast_pdf_${name}.png" "pdf_${name}.pdf" 2>/dev/null
  s=$(compare -metric RMSE "$c" "rast_svg_${name}.png" null: 2>&1 | sed 's/.*(\(.*\))/\1/')
  p=$(compare -metric RMSE "$c" "rast_pdf_${name}.png" null: 2>&1 | sed 's/.*(\(.*\))/\1/')
  printf "%-12s %-10s %-10s\n" "$name" "$s" "$p"
  awk -v v="$s" -v t="$TH" 'BEGIN{exit !(v>t)}' && { echo "  ^ SVG over threshold $TH"; fail=1; }
  awk -v v="$p" -v t="$TH" 'BEGIN{exit !(v>t)}' && { echo "  ^ PDF over threshold $TH"; fail=1; }
done
exit $fail
