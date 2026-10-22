FUNCTION Fib (n)
    IF n < 2 THEN
        Fib = n
    ELSE
        Fib = Fib(n - 1) + Fib(n - 2)
    END IF
END FUNCTION

FOR i = 0 TO 15
    PRINT "Fib("; i; ") = "; Fib(i)
NEXT
