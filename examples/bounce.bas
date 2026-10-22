CLS
x = 2: y = 2: dx = 1: dy = 1

FOR frame = 1 TO 300
    LOCATE y, x: PRINT " ";
    x = x + dx: y = y + dy
    IF x <= 1 OR x >= 60 THEN dx = -dx
    IF y <= 1 OR y >= 20 THEN dy = -dy
    COLOR frame MOD 6 + 9
    LOCATE y, x: PRINT "O";
    SLEEP 0.03
NEXT

COLOR
LOCATE 22, 1
