import QtCore
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// OSM consumer example (LP-0018): a third-party Basecamp app that embeds the
// osm SDK through the osm_consumer_example core module. Resolve a region
// from the on-chain registry, then download it with checksum verification.

ColumnLayout {
    id: root
    anchors.fill: parent
    anchors.margins: 16
    spacing: 10

    // Replica of the backend (osm_consumer_app.rep), served by the core
    // module osm-consumer-app_backend.
    readonly property var backend: logos.module("osm-consumer-app")

    Label {
        text: "OSM Consumer Example"
        font.bold: true
        font.pixelSize: 18
    }
    Label {
        text: "osm-consumer-app → osm_consumer_example → osm (SDK) → Logos Storage / LEZ registry"
        opacity: 0.7
    }

    RowLayout {
        Layout.fillWidth: true
        Label { text: "Region" }
        TextField {
            id: regionField
            Layout.preferredWidth: 260
            text: "china/henan"
        }
        Button {
            text: "Resolve"
            enabled: backend && !backend.working
            onClicked: backend.resolveRegion(regionField.text)
        }
        Button {
            text: "Download"
            enabled: backend && !backend.working
            onClicked: backend.downloadRegion(regionField.text,
                StandardPaths.writableLocation(StandardPaths.DownloadLocation).toString())
        }
        BusyIndicator {
            running: backend && backend.working
            visible: running
            implicitHeight: 28
        }
        Item { Layout.fillWidth: true }
    }

    Label {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        text: "Status: " + (backend ? backend.status : "connecting…")
    }
    Label {
        Layout.fillWidth: true
        visible: backend && backend.lastVerification !== ""
        font.bold: true
        color: backend && backend.lastVerification === "verified" ? "#1b7f3b" : "#b00020"
        text: backend ? (backend.lastVerification === "verified"
                         ? "Checksum verified against the on-chain registry: " + backend.lastPath
                         : "CHECKSUM MISMATCH: " + backend.lastPath) : ""
    }

    Label { text: "Registry entry"; font.bold: true }
    ScrollView {
        Layout.fillWidth: true
        Layout.fillHeight: true
        TextArea {
            readOnly: true
            font.family: "monospace"
            text: backend && backend.entry !== "" ? backend.entry : "(resolve a region)"
            wrapMode: TextEdit.WrapAnywhere
        }
    }
}
