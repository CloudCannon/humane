Feature: Base Tests

    Scenario: Tests can run
        Given I have a "peachy" file with the content:
            """
            Peach!
            """
        Then I should see "Peach!" in "peachy"
