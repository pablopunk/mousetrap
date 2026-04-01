// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Mousetrap",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(name: "Mousetrap", targets: ["Mousetrap"])
    ],
    targets: [
        .executableTarget(
            name: "Mousetrap",
            path: "Sources"
        )
    ]
)
