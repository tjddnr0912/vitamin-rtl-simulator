typedef struct packed { logic [1:0] hist; logic err; logic [3:0] index; } pkt_t;
module dff #(parameter WIDTH = 1) (input logic clk, input logic [WIDTH-1:0] din, output logic [WIDTH-1:0] dout);
  always_ff @(posedge clk) dout <= din;
endmodule
module top;
  logic clk = 0;
  pkt_t p0, p1;
  dff #(3) f (.clk(clk), .din(3'b101), .dout({p0.hist[1:0], p0.err}));     // concat of struct-member selects as an output actual
  dff #(8) g (.clk(clk), .din(8'hA5), .dout({p0.index[3:0], p1.index[3:0]}));
  dff #(3) h (.clk(clk), .din(3'b010), .dout({p1.hist, p1.err}));
  initial begin #1 clk = 1; #1 $display("p0=%b p1=%b", p0, p1); $finish; end
endmodule
