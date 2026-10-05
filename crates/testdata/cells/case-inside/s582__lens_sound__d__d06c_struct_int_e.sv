module top;
  typedef struct { int k; } us_t;
  us_t us; int m, t;
  initial begin
    us.k = -1;
    case (us.k) inside [-2:0]: m = 1; [1:2]: m = 2; default: m = 0; endcase
    t = us.k inside {[-2:0]};
    $display("E us.k=-1 m=%0d if-twin=%0d", m, t);
    #1 $finish;
  end
endmodule
