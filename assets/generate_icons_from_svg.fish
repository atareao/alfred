#!/usr/bin/env fish

# Comprobar que se ha pasado un archivo SVG como argumento
if test (count $argv) -eq 0
    echo "Error: Debes indicar el archivo SVG de origen."
    echo "Uso: ./generate_icons_from_svg.fish icono.svg"
    exit 1
end

set INPUT $argv[1]

if not test -f $INPUT
    echo "Error: El archivo '$INPUT' no existe."
    exit 1
end

# Verificar extensión SVG
if not string match -ri '\.svg$' $INPUT > /dev/null
    echo "Aviso: El archivo no tiene extensión .svg, intentando procesar de todos modos..."
end

echo "🎩 Procesando el SVG '$INPUT' para generar el kit de iconos de Valet..."

# Determinar comando de ImageMagick
set CONVERT_CMD "magick"
if not type -q magick
    set CONVERT_CMD "convert"
end

# Función auxiliar para renderizar desde SVG con densidad alta
function render_svg
    set -l src $argv[1]
    set -l size $argv[2]
    set -l dest $argv[3]
    
    # -density 300 asegura que el SVG se renderice nítido a cualquier resolución
    $CONVERT_CMD -background none -density 300 $src -resize {$size}x{$size} $dest
end

# -------------------------------------------------------------------
# 1. Favicons y Web Icons
# -------------------------------------------------------------------
echo "📁 Creando iconos Web / Favicons..."
mkdir -p frontend/public

render_svg $INPUT 16 frontend/public/favicon-16x16.png
render_svg $INPUT 32 frontend/public/favicon-32x32.png
render_svg $INPUT 48 frontend/public/favicon-48x48.png
render_svg $INPUT 180 frontend/public/apple-touch-icon.png
render_svg $INPUT 192 frontend/public/icon-192.png
render_svg $INPUT 512 frontend/public/icon-512.png

# Generar favicon.ico multiresolución
$CONVERT_CMD -background none -density 300 $INPUT -define icon:auto-resize=16,32,48 frontend/public/favicon.ico

# -------------------------------------------------------------------
# 2. Linux / Desktop / App Icons
# -------------------------------------------------------------------
echo "📁 Creando iconos de escritorio (Linux / App)..."
set LINUX_SIZES 16 24 32 48 64 96 128 256 512 1024

for size in $LINUX_SIZES
    mkdir -p assets/linux/hicolor/{$size}x{$size}/apps
    render_svg $INPUT $size assets/linux/hicolor/{$size}x{$size}/apps/valet.png
end

# Copiar también el SVG original para iconos escalables de Linux
mkdir -p assets/linux/hicolor/scalable/apps
cp $INPUT assets/linux/hicolor/scalable/apps/valet.svg

# -------------------------------------------------------------------
# 3. Android App Icons (Mipmap)
# -------------------------------------------------------------------
echo "📁 Creando iconos para Android (mipmaps)..."
set -l android_dirs  "mipmap-mdpi" "mipmap-hdpi" "mipmap-xhdpi" "mipmap-xxhdpi" "mipmap-xxxhdpi"
set -l android_sizes 48            72            96              144              192

for i in (seq (count $android_dirs))
    mkdir -p assets/android/$android_dirs[$i]
    render_svg $INPUT $android_sizes[$i] assets/android/$android_dirs[$i]/ic_launcher.png
end

# -------------------------------------------------------------------
# 4. iOS App Icons
# -------------------------------------------------------------------
echo "📁 Creando iconos para iOS / App Store..."
mkdir -p assets/ios

set -l ios_files "AppIcon-20x20@2x.png" "AppIcon-20x20@3x.png" "AppIcon-29x29@2x.png" "AppIcon-29x29@3x.png" "AppIcon-40x40@2x.png" "AppIcon-40x40@3x.png" "AppIcon-60x60@2x.png" "AppIcon-60x60@3x.png" "iTunesArtwork@2x.png"
set -l ios_sizes 40                      60                      58                      87                      80                      120                     120                     180                     1024

for i in (seq (count $ios_files))
    render_svg $INPUT $ios_sizes[$i] assets/ios/$ios_files[$i]
end

echo "✨ ¡Listo! Todos los PNG e ICO han sido exportados limpiamente desde tu SVG."
