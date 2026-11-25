import QtQuick
import QtQuick.Controls

// An inset text field on the brand tokens (FRAGO's Field, mono: it holds paths).
TextField {
    id: f
    font.family: Theme.mono
    font.pixelSize: Theme.fData
    color: Theme.ink
    placeholderTextColor: Theme.dim
    selectionColor: Theme.wash(0.5)
    selectedTextColor: Theme.ink
    leftPadding: 8
    rightPadding: 8
    topPadding: 0
    bottomPadding: 0
    verticalAlignment: TextInput.AlignVCenter
    implicitHeight: Theme.row
    background: Rectangle {
        color: Theme.field
        radius: Theme.radius
        border.width: 1
        border.color: f.activeFocus ? Theme.sand : Theme.rule
    }
}
