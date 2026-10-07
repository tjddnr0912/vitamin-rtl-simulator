`timescale 1ns/1ps
module r01b;
reg clk = 1'b0;
reg [7:0] q = 8'd0, q_next;
reg s = 1'b1, l = 1'b1;
wire e;
assign e = s && !l;           // computed wire: x -> 0 at t0
always @* begin
  q_next = q;                  // reads its own register and the computed wire
  if (e) q_next = 8'd7;
end
always @(posedge clk) q <= q_next;
initial begin
  #1 $display("t1 e=%b q_next=%h", e, q_next);
  clk = 1'b1;
  #1 $display("t2 q=%h", q);
  $finish;
end
endmodule
