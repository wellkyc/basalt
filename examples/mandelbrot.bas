CONST MAXITER = 40
shades$ = " .:-=+*#%@"

FOR y = -12 TO 12
    row$ = ""
    FOR x = -39 TO 38
        cr = x / 22 - 0.6
        ci = y / 11
        zr = 0: zi = 0: n = 0
        DO WHILE zr * zr + zi * zi < 4 AND n < MAXITER
            t = zr * zr - zi * zi + cr
            zi = 2 * zr * zi + ci
            zr = t
            n = n + 1
        LOOP
        row$ = row$ + MID$(shades$, INT(n / MAXITER * 9) + 1, 1)
    NEXT
    PRINT row$
NEXT
