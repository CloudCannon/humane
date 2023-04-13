Feature: Base Tests

    Scenario: Tests can run
        Given I have a "peachy" file with the content:
            """
            Peach!
            """
        Then I should see "Peach!" in "peachy"

    Scenario: Commands can run
        When I run "echo Hello"
        Then I should see "Hello" in stdout

    Scenario: Commands get substitutions
        When I run "ls {{humane_cwd}}"
        Then I should see "Cargo.lock" in stdout
        Given I have a "peachy" file with the content:
            """
            Peach!
            """
        When I run "ls {{humane_temp_dir}}"
        Then I should see "peachy" in stdout
