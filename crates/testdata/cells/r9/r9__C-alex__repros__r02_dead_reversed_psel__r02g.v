`timescale 1ns/1ps
module r02g;
parameter N = 1;
localparam CL = $clog2(N);
localparam W = 8;
wire [W-1:0] tid;
reg [CL:0] g = 0;
assign tid[W-CL-1:0] = 8'h5a;
generate if (N > 1) begin : gx
  assign tid[W-1:W-CL] = g;            // dead generate branch: [7:8]
end endgenerate
initial #1 $display("tid=%h", tid);
endmodule
