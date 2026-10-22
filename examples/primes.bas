CONST LIMIT = 200
DIM composite(LIMIT)
count = 0

FOR i = 2 TO LIMIT
    IF composite(i) = 0 THEN
        count = count + 1
        PRINT RIGHT$("     " + STR$(i), 5);
        IF count MOD 10 = 0 THEN PRINT
        FOR j = i * i TO LIMIT STEP i
            composite(j) = 1
        NEXT
    END IF
NEXT

PRINT
PRINT count; " primes up to "; LIMIT
