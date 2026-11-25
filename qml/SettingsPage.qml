import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// SETTINGS: where profiles live, which Options dir is written, the theme.
Rectangle {
    id: page
    property string theme: "deepfield"
    signal themeChosen(string name)

    color: Theme.pane

    component Rule: Rectangle { width: parent ? parent.width : 0; height: 1; color: Theme.rule }

    // One setting: a label column, then whatever the row holds.
    component Setting: Item {
        id: setting
        property string label: ""
        default property alias content: slot.data
        width: parent ? parent.width : 0
        height: Theme.row + 10
        Kicker { x: 10; width: 190; height: parent.height; text: setting.label }
        Row { id: slot; x: 210; anchors.verticalCenter: parent.verticalCenter; spacing: 8 }
        Rule { anchors.bottom: parent.bottom }
    }
    component Value: Text {
        height: Theme.row
        verticalAlignment: Text.AlignVCenter
        font.family: Theme.mono
        font.pixelSize: Theme.fData
        color: Theme.ink
    }
    // One of a set: lit when it is the current choice.
    component Choice: Rectangle {
        id: choice
        property string label: ""
        property bool on: false
        signal picked()
        width: Math.max(96, ct.implicitWidth + 28)
        height: Theme.row
        radius: Theme.radius
        color: on ? Theme.wash(0.16) : (ch.hovered ? Theme.hover(0.06) : "transparent")
        border.width: 1
        border.color: on ? Theme.sand : Theme.rule
        Text {
            id: ct
            anchors.centerIn: parent
            text: choice.label
            font.family: Theme.cond
            font.pixelSize: Theme.fLabel + 2
            font.weight: Font.DemiBold
            font.letterSpacing: 1.2
            color: choice.on ? Theme.sandHi : Theme.ink
        }
        HoverHandler { id: ch; cursorShape: Qt.PointingHandCursor }
        TapHandler { onTapped: choice.picked() }
    }

    Column {
        anchors.fill: parent

        PaneHead { width: parent.width; title: "FOLDERS" }
        Setting {
            label: "PROFILES FOLDER"
            Field {
                id: profilesField
                width: Math.max(320, page.width - 210 - 240)
                text: Core.profilesDir
                onEditingFinished: Core.profilesDir = text
            }
            Btn { label: "BROWSE"; onClicked: Core.browseProfilesDir() }
            Btn { label: "DEFAULT"; live: Core.profilesDir !== Core.defaultProfilesDir; onClicked: Core.profilesDir = "" }
        }
        Setting {
            label: "OPTIONS DIR OVERRIDE"
            Field {
                width: Math.max(320, page.width - 210 - 240)
                text: Core.optionsOverride
                placeholderText: "none: " + Core.defaultOptionsDir
                onEditingFinished: Core.optionsOverride = text
            }
            Btn { label: "BROWSE"; onClicked: Core.browseOptionsDir() }
            Btn { label: "CLEAR"; live: Core.optionsOverride !== ""; onClicked: Core.optionsOverride = "" }
        }
        Setting {
            label: "OPTIONS DIR IN USE"
            Value {
                width: page.width - 220
                elide: Text.ElideMiddle
                text: Core.optionsDir + (Core.optionsOk ? "" : "   NOT FOUND")
                color: Core.optionsOk ? Theme.ink : Theme.crit
            }
        }
        Setting {
            label: "BACKUPS"
            Value {
                width: page.width - 220
                elide: Text.ElideMiddle
                text: Core.optionsDir + "\\_loadout-backup\\<time>   (one folder per APPLY, with manifest.toml)"
                color: Theme.dim
            }
        }

        PaneHead { width: parent.width; title: "APPEARANCE" }
        Setting {
            label: "THEME"
            Choice { label: "DEEPFIELD"; on: page.theme !== "daylight"; onPicked: page.themeChosen("deepfield") }
            Choice { label: "DAYLIGHT"; on: page.theme === "daylight"; onPicked: page.themeChosen("daylight") }
        }

        PaneHead { width: parent.width; title: "ABOUT" }
        Setting { label: "VERSION"; Value { text: BuildInfo.version } }
        Setting { label: "BUILT"; Value { text: BuildInfo.built + "   @ " + BuildInfo.sha } }
        Setting {
            label: "COMMAND LINE"
            Value {
                width: page.width - 220
                elide: Text.ElideRight
                text: "ed-loadout.exe, beside loadout.exe: the same core, for scripting"
                color: Theme.dim
            }
        }
    }
}
