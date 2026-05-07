ios:
    open /Applications/Xcode.app/Contents/Developer/Applications/Simulator.app
    xcrun simctl boot "iPhone 17"
    cd mobile && dx serve --ios
