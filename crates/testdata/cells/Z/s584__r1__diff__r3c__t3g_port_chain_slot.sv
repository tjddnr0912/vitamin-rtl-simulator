module mm(input logic [1:0] i, output logic [1:0] o);
  always_comb o = i;
endmodule
module mc(input logic [1:0] i, output logic [1:0] y);
  always_comb begin unique case (i) 2'd1: y = 2'd1; 2'd2: y = 2'd2; endcase end
endmodule
module top;
  logic [1:0] src = 2'd1;
  logic [1:0] p;
  wire [1:0] m, y;
  always_comb p = src;
  mm u_m(.i(p), .o(m));
  mc u_c(.i(m), .y(y));
  initial #1 $display("t=%0t y=%0d", $time, y);
  initial #5 $finish;
endmodule
