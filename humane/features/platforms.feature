Feature: Platform Flags

    @platform-unix
    Scenario: Unix platform tag
        Given I have an "output" file with the content:
            """
            I am Linux or MacOS!
            """
        Then I should see "I am Linux or MacOS!" in "output"

    @platform-linix
    Scenario: Linux platform tag
        Given I have an "output" file with the content:
            """
            I am Linux!
            """
        Then I should see "I am Linux!" in "output"

    @platform-macos
    Scenario: macOS platform tag
        Given I have an "output" file with the content:
            """
            I am macOS!
            """
        Then I should see "I am macOS!" in "output"

    @platform-windows
    Scenario: Windows platform tag
        Given I have an "output" file with the content:
            """
            I am Windows!
            """
        Then I should see "I am Windows!" in "output"
