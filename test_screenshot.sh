#!/bin/bash

echo "================================"
echo "Testing screenshot functionality"
echo "================================"
echo ""

# Test scrot
if command -v scrot &> /dev/null; then
    echo "✓ scrot found"
    scrot /tmp/test_scrot.png --overwrite 2>&1
    if [ -f /tmp/test_scrot.png ]; then
        SIZE=$(stat -f%z /tmp/test_scrot.png 2>/dev/null || stat -c%s /tmp/test_scrot.png 2>/dev/null)
        echo "✓ scrot works (${SIZE} bytes)"
        rm /tmp/test_scrot.png
    else
        echo "✗ scrot failed to create file"
    fi
else
    echo "✗ scrot not found"
fi

echo ""

# Test gnome-screenshot
if command -v gnome-screenshot &> /dev/null; then
    echo "✓ gnome-screenshot found"
    gnome-screenshot -f /tmp/test_gnome.png 2>&1
    if [ -f /tmp/test_gnome.png ]; then
        SIZE=$(stat -f%z /tmp/test_gnome.png 2>/dev/null || stat -c%s /tmp/test_gnome.png 2>/dev/null)
        echo "✓ gnome-screenshot works (${SIZE} bytes)"
        rm /tmp/test_gnome.png
    else
        echo "✗ gnome-screenshot failed to create file"
    fi
else
    echo "✗ gnome-screenshot not found"
fi

echo ""

# Test imagemagick
if command -v import &> /dev/null; then
    echo "✓ imagemagick found"
    import -window root /tmp/test_im.png 2>&1
    if [ -f /tmp/test_im.png ]; then
        SIZE=$(stat -f%z /tmp/test_im.png 2>/dev/null || stat -c%s /tmp/test_im.png 2>/dev/null)
        echo "✓ imagemagick works (${SIZE} bytes)"
        rm /tmp/test_im.png
    else
        echo "✗ imagemagick failed to create file"
    fi
else
    echo "✗ imagemagick not found"
fi

echo ""
echo "================================"
echo "Test Summary"
echo "================================"
echo "At least ONE method should work for screenshot functionality."
echo "On Ubuntu: scrot is recommended (fastest)"
echo ""
