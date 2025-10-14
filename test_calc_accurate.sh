#!/bin/bash
# Accurate Calculator button click test
# Uses wmctrl for reliable window geometry

set -e

echo "=== Accurate Calculator Click Test ==="
echo ""
echo "🎯 This test will click the actual Calculator buttons"
echo "   WATCH YOUR SCREEN to see if '123' appears!"
echo ""

# Get Calculator window with wmctrl (more reliable than xdotool for geometry)
echo "1. Finding Calculator window..."
CALC_GEOM=$(wmctrl -lG | grep -i calculator | head -1)

if [ -z "$CALC_GEOM" ]; then
    echo "   Calculator not found, launching..."
    gio launch /usr/share/applications/org.gnome.Calculator.desktop &
    sleep 3
    CALC_GEOM=$(wmctrl -lG | grep -i calculator | head -1)
fi

if [ -z "$CALC_GEOM" ]; then
    echo "❌ ERROR: Cannot find Calculator window!"
    exit 1
fi

echo "   Found: $CALC_GEOM"

# Parse geometry: format is "WINDOW DESKTOP X Y WIDTH HEIGHT HOST TITLE"
X=$(echo "$CALC_GEOM" | awk '{print $3}')
Y=$(echo "$CALC_GEOM" | awk '{print $4}')
WIDTH=$(echo "$CALC_GEOM" | awk '{print $5}')
HEIGHT=$(echo "$CALC_GEOM" | awk '{print $6}')

echo "   Position: ($X, $Y)"
echo "   Size: ${WIDTH}x${HEIGHT}"

# Calculate button positions for GNOME Calculator
# Standard GNOME Calculator layout:
#   - Display at top (~100px)
#   - Number pad in 4x4 grid below
#   - Row 1 (from bottom): 0 . =
#   - Row 2: 1 2 3 +
#   - Row 3: 4 5 6 -
#   - Row 4: 7 8 9 ×
#   - Buttons are roughly 1/4 width and spaced evenly

# Button row calculations (from bottom up)
BUTTON_WIDTH=$((WIDTH / 4))
BUTTON_HEIGHT=$((HEIGHT / 6))

# Row 2 from bottom (buttons 1, 2, 3, +)
BUTTON_ROW_Y=$((Y + HEIGHT - (2 * BUTTON_HEIGHT) - (BUTTON_HEIGHT / 2)))

# Button columns (1, 2, 3)
BUTTON_1_X=$((X + (BUTTON_WIDTH / 2)))           # Column 1
BUTTON_2_X=$((X + BUTTON_WIDTH + (BUTTON_WIDTH / 2)))  # Column 2
BUTTON_3_X=$((X + (2 * BUTTON_WIDTH) + (BUTTON_WIDTH / 2)))  # Column 3

echo ""
echo "2. Calculated button positions:"
echo "   Button '1': ($BUTTON_1_X, $BUTTON_ROW_Y)"
echo "   Button '2': ($BUTTON_2_X, $BUTTON_ROW_Y)"
echo "   Button '3': ($BUTTON_3_X, $BUTTON_ROW_Y)"

# Activate Calculator window
echo ""
echo "3. Activating Calculator window..."
WINDOW_ID=$(echo "$CALC_GEOM" | awk '{print $1}')
wmctrl -i -a "$WINDOW_ID"
sleep 0.5

# Clear calculator (press C key)
echo "4. Clearing calculator..."
xdotool key c
sleep 0.5

echo ""
echo "=== CLICKING BUTTONS NOW ==="
echo "   👀 WATCH THE CALCULATOR DISPLAY!"
echo ""
sleep 1

# Click button 1
echo "5. Clicking '1'..."
xdotool mousemove $BUTTON_1_X $BUTTON_ROW_Y
sleep 0.3
xdotool click 1
sleep 0.8

# Click button 2
echo "6. Clicking '2'..."
xdotool mousemove $BUTTON_2_X $BUTTON_ROW_Y
sleep 0.3
xdotool click 1
sleep 0.8

# Click button 3
echo "7. Clicking '3'..."
xdotool mousemove $BUTTON_3_X $BUTTON_ROW_Y
sleep 0.3
xdotool click 1
sleep 1

echo ""
echo "=== TEST COMPLETE ==="
echo ""
echo "📊 Result Check:"
echo "   ✅ All click commands executed successfully"
echo ""
echo "🔍 Visual Verification:"
echo "   Look at the Calculator display now."
echo ""
echo "   ❓ Do you see '123' displayed?"
echo ""
echo "   ✅ YES → Clicks are working! Bridge should work identically."
echo "   ❌ NO  → Clicks not registering. Possible issues:"
echo "             - Window not focused"
echo "             - Button positions incorrect"
echo "             - Display lag in remote session"
echo ""
echo "💡 Next step: Test the BRIDGE with same coordinates via gRPC"
