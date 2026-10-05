module top;
  typedef struct { string s; int k; } us_t;
  us_t us; int m;
  initial begin
    us.s = "b"; us.k = 1;
    case (us.s) inside "b": m = 1; "a": m = 2; default: m = 0; endcase
    $display("A us.s=b m=%0d", m);
    us.s = "a";
    case (us.s) inside "b": m = 1; "a": m = 2; default: m = 0; endcase
    $display("B us.s=a m=%0d", m);
    us.s = "abc";
    case (us.s) inside "b": m = 1; "a": m = 2; default: m = 0; endcase
    $display("C us.s=abc m=%0d", m);
    case (us.k) inside [1:2]: m = 1; default: m = 0; endcase
    $display("D us.k=1 m=%0d", m);
    us.k = -1;
    case (us.k) inside [-2:0]: m = 1; [1:2]: m = 2; default: m = 0; endcase
    $display("E us.k=-1 m=%0d", m);
    #1 $finish;
  end
endmodule
