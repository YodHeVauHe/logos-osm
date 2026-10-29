import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

// OSM distribution app (LP-0018).
//
// Left: the frozen predefined region set (country / subregion) with hosted
// status and versions. Right: actions — bulk host with per-region opt-out,
// local import with checksum verification, download with verification
// result, and an update banner when Geofabrik has newer snapshots.

ColumnLayout {
    id: root
    anchors.fill: parent
    spacing: 8

    // ── state ────────────────────────────────────────────────────────────
    property var regionList: {
        try { return JSON.parse(appBridge.regions) } catch (e) { return [] }
    }
    property var entryMap: {
        const rows = JSON.parse(appBridge.registryEntries || "[]")
        const entries = ({})
        for (const entry of rows)
            if (entry && entry.region) entries[entry.region] = entry
        return entries
    }
    property var updateList: {
        try { return JSON.parse(appBridge.updates) } catch (e) { return [] }
    }
    property bool overlapWarning: {
        // the frozen set is non-overlapping; warn only if the module is
        // ever extended beyond it
        return false
    }

    Connections {
        target: appBridge
        function onStatusChanged(s) { statusLine.text = s }
        function onLastVerificationChanged(v) { verifyLine.text = v === "" ? "" : (v === "verified" ? "checksum verified" : "CHECKSUM MISMATCH") }
    }

    // ── header ───────────────────────────────────────────────────────────
    RowLayout {
        Layout.fillWidth: true
        Label {
            text: "OpenStreetMap Distribution"
            font.bold: true
            font.pixelSize: 18
        }
        Item { Layout.fillWidth: true }
        Button {
            text: "Check updates"
            enabled: !appBridge.working
            onClicked: appBridge.checkUpdates()
        }
        Button {
            text: "Refresh"
            enabled: !appBridge.working
            onClicked: root.refreshEntries()
        }
    }

    Banner {
        id: updateBanner
        Layout.fillWidth: true
        visible: root.updateList.length > 0
        text: root.updateList.length + " region(s) have newer Geofabrik snapshots available"
        actionText: "Re-host"
        onAction: {
            const names = root.updateList.map(u => u.region).join(",")
            appBridge.hostRegions(names)
        }
    }

    Label {
        id: statusLine
        Layout.fillWidth: true
        text: appBridge.status
        color: appBridge.phase === "error" ? "#c0392b" : undefined
        elide: Text.ElideRight
    }
    Label {
        id: verifyLine
        Layout.fillWidth: true
        text: ""
        visible: text !== ""
        color: text.indexOf("MISMATCH") >= 0 ? "#c0392b" : "#1e8e3e"
        font.bold: true
    }

    // ── bulk host toolbar ────────────────────────────────────────────────
    RowLayout {
        Layout.fillWidth: true
        Button {
            text: "Host selected"
            enabled: !appBridge.working && root.selectedCount > 0
            onClicked: {
                const names = []
                for (const r of root.regionList)
                    if (root.selection[r.region]) names.push(r.region)
                appBridge.hostRegions(names.join(","))
            }
        }
        Button {
            text: "Select all"
            onClicked: root.selectAll(true)
        }
        Button {
            text: "Clear"
            onClicked: root.selectAll(false)
        }
        Label { text: root.selectedCount + " selected" }
    }

    // ── region table ─────────────────────────────────────────────────────
    ListView {
        id: regionView
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        model: root.regionList
        delegate: Rectangle {
            width: regionView.width
            height: 34
            color: index % 2 === 0 ? "transparent" : Qt.rgba(0.5, 0.5, 0.5, 0.08)

            RowLayout {
                anchors.fill: parent
                anchors.margins: 4
                spacing: 8
                CheckBox {
                    checked: root.selection[modelData.region] === true
                    onToggled: {
                        root.selection[modelData.region] = checked
                        root.selectionChanged()
                    }
                }
                Label {
                    Layout.preferredWidth: 240
                    text: modelData.region
                    font.family: "monospace"
                    elide: Text.ElideMiddle
                }
                Badge {
                    text: modelData.level === 0 ? "country" : "subregion"
                }
                Label {
                    Layout.preferredWidth: 140
                    text: {
                        const e = root.entryMap[modelData.region]
                        if (!e) return "not hosted"
                        return (e.hosted ? "hosted" : "registered") + " · " + e.version
                    }
                    color: {
                        const e = root.entryMap[modelData.region]
                        return e && e.hosted ? "#1e8e3e" : "#888"
                    }
                }
                Item { Layout.fillWidth: true }
                Button {
                    text: "Download"
                    enabled: !appBridge.working
                    onClicked: appBridge.downloadRegion(modelData.region, StandardPaths.writableLocation(StandardPaths.DownloadLocation))
                }
            }
        }
        ScrollIndicator.vertical: ScrollIndicator {}
    }

    // ── import row ───────────────────────────────────────────────────────
    RowLayout {
        Layout.fillWidth: true
        Label { text: "Import local PBF:" }
        Button {
            text: "Choose file…"
            enabled: !appBridge.working
            onClicked: importDialog.open()
        }
        Label { id: importPathLabel; Layout.fillWidth: true; elide: Text.ElideMiddle; color: "#888" }
    }

    FileDialog {
        id: importDialog
        nameFilters: ["OpenStreetMap PBF (*.osm.pbf)"]
        onAccepted: {
            importPathLabel.text = selectedFile.toString().replace("file://", "")
            importRegionPicker.visible = true
        }
    }

    Dialog {
        id: importRegionPicker
        title: "Which predefined region is this file?"
        standardButtons: Dialog.Ok | Dialog.Cancel
        visible: false
        ComboBox {
            id: regionCombo
            model: root.regionList.map(r => r.region)
            Layout.fillWidth: true
        }
        onAccepted: {
            if (importPathLabel.text !== "")
                appBridge.importRegion(regionCombo.currentText, importPathLabel.text, "")
        }
    }

    // ── helpers ──────────────────────────────────────────────────────────
    property var selection: ({})
    property int selectedCount: Object.keys(root.selection).filter(k => root.selection[k]).length
    function selectAll(v) {
        const s = {}
        for (const r of root.regionList) s[r.region] = v
        root.selection = s
        root.selectionChanged()
    }
    function refreshEntries() {
        // Entry state comes from the osm module through the app bridge.
        appBridge.refresh()
    }

    component Badge : Rectangle {
        property string text
        implicitWidth: label.implicitWidth + 12
        implicitHeight: 18
        radius: 9
        color: Qt.rgba(0.4, 0.4, 0.4, 0.25)
        Label { id: label; anchors.centerIn: parent; text: parent.text; font.pixelSize: 11 }
    }

    component Banner : Rectangle {
        property string text
        property string actionText
        signal action()
        Layout.preferredHeight: 40
        radius: 6
        color: "#4a3b12"
        RowLayout {
            anchors.fill: parent
            anchors.margins: 8
            Label { text: parent.parent.text; Layout.fillWidth: true; color: "#ffd75e" }
            Button { text: parent.parent.actionText; onClicked: parent.parent.action() }
        }
    }
}
