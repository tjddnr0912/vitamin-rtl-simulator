`timescale 1ns/1ps
module r01;
reg clk = 1'b0;
reg [7:0] q = 8'd0, q_next;
reg [7:0] a = 8'd5;
reg [7:0] b_next;
always @* q_next = q;          // reads only its own register
always @* b_next = a + 8'd1;   // reads a declaration-initialised variable
always @(posedge clk) q <= q_next;
initial begin
  #1 $display("t1 q_next=%h b_next=%h", q_next, b_next);
  clk = 1'b1;
  #1 $display("t2 q=%h", q);
  $finish;
end
endmodule
