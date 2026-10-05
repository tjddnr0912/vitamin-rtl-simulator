module top;
  logic [1:0] s = 2'd1;
  wire [1:0] #0 ws = s;
  logic [1:0] p, m, y;
  always_comb p = ws;
  always_comb m = p;
  always_comb begin unique case (m) 2'd1: y = 2'd1; 2'd2: y = 2'd2; endcase end
  initial #1 $display("t=%0t y=%0d", $time, y);
  initial #5 $finish;
endmodule
