`timescale 1ns/1ns
module t;
  logic [7:0] v; logic [3:0] r;
  case (1'b1) (4'b1100 inside {4'b1?00}): begin : gc1 initial #1 $display("X3 gencase item"); end default: begin : gc2 initial #1 $display("X3 gencase default"); end endcase
  initial begin
    v = 8'hFF;
    r = {((4'b1100 inside {4'b1?00}) ? 2 : 3){1'b1}};
    $display("X3 rep %b", r);
    $display("X3 psel %b", v[((4'b1100 inside {4'b1?00}) ? 1 : 2) : 0]);
    #2 $finish;
  end
endmodule
