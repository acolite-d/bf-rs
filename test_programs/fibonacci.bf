,                    Read into c0
>+>+                 c1 = 1; c2 = 1
<<                   Move back to c0
[                    Main loop start
  ->>                Decrement c0 move to c2
  [->+>+<<]          Add c2 to c3 and c4; c2 = 0
  <                  Move to c1
  [->>+<<]           Add c1 to c3; c1 = 0
  >>                 Move to c3
  [-<+>]             Add c3 to c2; c3 = 0
  >                  Move to c4
  [-<<<+>>>]         Move c4 to c1
  <<<<               Move to c0
]                    End main loop; jump back if c0 != 0
>>                   Result is in c2
.                    Print it
