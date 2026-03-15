#!/bin/bash
# macOS post-install: remind user to grant Accessibility permissions for paste simulation
osascript -e 'display notification "Please grant Accessibility access in System Settings > Privacy & Security > Accessibility to enable paste simulation." with title "Pisum Transcript Installed"'
