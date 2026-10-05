module top;
  typedef struct { string s; int k; } us_t;
  string sarr [2]; string sq [$]; us_t us; real sr; real rt; string sm [int];
  int m;
  initial begin
    sarr[0] = "a"; sarr[1] = "b"; sq.push_back("b"); us.s = "b"; us.k = 1; sr = 1.0; rt = 2.0; sm[3] = "b";
    case (sarr[1]) inside "b": m = 1; "a": m = 2; default: m = 0; endcase   // L8
    $display("L8 m=%0d", m);
    case (sq[0]) inside "b": m = 1; default: m = 0; endcase                // L10
    $display("L10 m=%0d", m);
    case (us.s) inside "b": m = 1; default: m = 0; endcase                 // L12
    $display("L12 m=%0d", m);
    case (sr) inside 1: m = 1; default: m = 0; endcase                     // L14
    $display("L14 m=%0d", m);
    case (rt) inside 2: m = 1; default: m = 0; endcase                     // L16
    $display("L16 m=%0d", m);
    case (sm[3]) inside "b": m = 1; default: m = 0; endcase                // L18
    $display("L18 m=%0d", m);
    case (us.k) inside [1:2]: m = 1; default: m = 0; endcase               // L20
    $display("L20 m=%0d", m);
    #1 $finish;
  end
endmodule
