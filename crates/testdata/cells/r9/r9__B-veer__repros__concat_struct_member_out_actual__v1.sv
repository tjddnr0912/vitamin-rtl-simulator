typedef struct packed { logic [1:0] hist; logic err; logic [3:0] index; } pkt_t;
module dff #(parameter WIDTH = 1) (input logic clk, input logic [WIDTH-1:0] din, output logic [WIDTH-1:0] dout);
  always_ff @(posedge clk) dout <= din;
endmodule
module top;
  logic clk = 0;
  pkt_t p0, p1;
  dff #(4) g (.clk(clk), .din(4'hA), .dout(p0.index[3:0]));   // member + part-select, no concat
  assign p0.hist[1:0] = 2'b11;                                 // continuous assign, member + part-select
  assign p0.err = 1'b0;
  initial begin p1.index[3:0] = 4'h5; #1 clk = 1; #1 $display("p0=%b p1=%b", p0, p1); $finish; end
endmodule
