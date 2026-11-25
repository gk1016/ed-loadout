import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// LOG: the lines the core printed for what this window ran - the same lines
// ed-loadout.exe prints for the same command.
Rectangle {
    id: page
    color: Theme.pane

    function tone(line) {
        if (line.startsWith("error") || line.startsWith("refusing")) return Theme.crit
        if (line.startsWith("warning") || line.startsWith("notice") || line.startsWith("note")) return Theme.warn
        if (line.startsWith("> ")) return Theme.ink
        return Theme.dim
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        PaneHead {
            Layout.fillWidth: true
            title: "LOG"
            note: Core.log.length + (Core.log.length === 1 ? " line" : " lines")
        }
        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            model: Core.log
            // Always shown when the lines overflow: a log is read by scrolling.
            ScrollBar.vertical: ScrollBar {
                policy: list.contentHeight > list.height ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff
                contentItem: Rectangle { implicitWidth: 6; color: Theme.ruleHi; radius: 0 }
            }
            onCountChanged: Qt.callLater(list.positionViewAtEnd)
            delegate: Rectangle {
                id: row
                required property string modelData
                required property int index
                width: list.width
                height: Theme.row
                // A command starts a block.
                color: modelData.startsWith("> ") ? Theme.rowAlt : "transparent"
                Text {
                    x: 10
                    width: 44
                    height: parent.height
                    verticalAlignment: Text.AlignVCenter
                    text: row.index + 1
                    font.family: Theme.mono
                    font.pixelSize: Theme.fMicro
                    color: Theme.ghost
                }
                Text {
                    x: 58
                    width: parent.width - 68
                    height: parent.height
                    verticalAlignment: Text.AlignVCenter
                    text: row.modelData
                    elide: Text.ElideRight
                    font.family: Theme.mono
                    font.pixelSize: Theme.fData
                    color: page.tone(row.modelData)
                }
                Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.rule }
            }
            Text {
                visible: list.count === 0
                x: 10
                width: parent.width - 20
                height: Theme.row
                verticalAlignment: Text.AlignVCenter
                text: "Nothing has run yet. DETECT AGAIN, CAPTURE, APPLY and RESTORE write their lines here."
                font.family: Theme.sans
                font.pixelSize: Theme.fBody
                color: Theme.dim
            }
        }
        Rectangle {
            Layout.fillWidth: true
            height: Theme.row + 12
            color: Theme.header
            Rectangle { anchors.top: parent.top; width: parent.width; height: 1; color: Theme.rule }
            Btn {
                x: 10
                anchors.verticalCenter: parent.verticalCenter
                label: "CLEAR"
                live: Core.log.length > 0
                onClicked: Core.clearLog()
            }
        }
    }
}
