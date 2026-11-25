import QtQuick

// A section head or a column label: Barlow Condensed, small, tracked, faint.
Text {
    font.family: Theme.cond
    font.pixelSize: Theme.fLabel + 1
    font.weight: Font.DemiBold
    font.letterSpacing: 1.4
    color: Theme.faint
    elide: Text.ElideRight
    verticalAlignment: Text.AlignVCenter
}
