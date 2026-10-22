10 PRINT "Counting down the classic way"
20 n = 5
30 PRINT n; "...";
40 n = n - 1
50 IF n > 0 THEN 30
60 PRINT "liftoff!"
70 GOSUB 100
80 END
100 PRINT "GOSUB and RETURN work too"
110 RETURN
