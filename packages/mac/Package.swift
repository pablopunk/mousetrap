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
    dependencies: [
        .package(url: "https://github.com/sindresorhus/KeyboardShortcuts", from: "2.0.0")
    ],
    targets: [
        .executableTarget(
            name: "Mousetrap",
            dependencies: [
                .product(name: "KeyboardShortcuts", package: "KeyboardShortcuts")
            ],
            path: "Sources"
        )
    ]
)
