typedef struct packed { logic [7:0] W; logic [7:0] D; } prm_t;   // 80-bit packed-struct parameter type
module dff #(parameter W = 4) (input logic clk, input logic [W-1:0] d, output logic [W-1:0] q);
  always_ff @(posedge clk) q <= d;
endmodule
module top #(parameter prm_t pt = 16'h0402);   // W=4, D=2
  logic clk = 0;
  logic [pt.D-1:0][pt.W-1:0] a, b, c;
  for (genvar i = 0; i < pt.D; i++) begin : g
    dff #(pt.W) f (.clk(clk), .d(pt.W'(i + 3)), .q(a[i][pt.W-1:0]));   // output actual
    assign b[i][pt.W-1:0] = pt.W'(i + 5);                                 // continuous assign
  end
  initial begin
    c[1][pt.W-1:0] = 4'h9;                                               // procedural, constant index
    c[0][pt.W-1:0] = 4'h8;
    #1 clk = 1; #1;
    $display("W=%0d D=%0d a=%h b=%h c=%h", pt.W, pt.D, a, b, c);
    $finish;
  end
endmodule
