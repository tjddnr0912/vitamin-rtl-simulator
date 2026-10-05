package pa; localparam P = 3; endpackage
module top;
  import pa::P;
  typedef struct packed { logic [3:0] P; logic [3:0] Q; } s_t;
  s_t s;
  initial begin s.P = 4'd9; s.Q = 4'd1; #1 $display("d13 P=%0d sP=%0d", P, s.P); end
  initial #100 $finish;
endmodule
