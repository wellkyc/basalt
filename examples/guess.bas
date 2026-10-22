RANDOMIZE TIMER
target = INT(RND * 100) + 1
tries = 0

PRINT "I'm thinking of a number between 1 and 100."
DO
    INPUT "Your guess"; guess
    tries = tries + 1
    IF guess < target THEN
        PRINT "Higher!"
    ELSEIF guess > target THEN
        PRINT "Lower!"
    END IF
LOOP UNTIL guess = target

PRINT "Correct! You got it in "; tries; " tries."
