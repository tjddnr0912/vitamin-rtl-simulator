`timescale 1ns/1ps
module r02t;
parameter N = 1;
localparam CL = $clog2(N);
localparam W = 8;
reg [W-1:0] tid;
reg [CL:0] g = 0;
always @(g) begin
  tid = 8'h5a;
  tid[W-1:W-CL] = (N > 1) ? g : tid[W-1:W-CL];   // reachable reversed select: [7:8]
end
initial begin #1 g = 1; #1 $display("tid=%h", tid); end
endmodule
