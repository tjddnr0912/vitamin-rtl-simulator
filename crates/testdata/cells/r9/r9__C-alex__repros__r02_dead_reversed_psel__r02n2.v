`timescale 1ns/1ps
module r02;
parameter N = 2;                       // axis_switch / axis_arb_mux / axis_ram_switch with S_COUNT=1
localparam CL = $clog2(N);             // 0
localparam W = 8;
reg [W-1:0] tid;
reg [CL:0] g = 0;
always @(g) begin
  tid = 8'h5a;
  if (N > 1) begin                     // constant-false: the select below is never executed
    tid[W-1:W-CL] = g;                 // [7:8] when CL == 0
  end
end
initial begin #1 g = 1; #1 $display("tid=%h", tid); end
endmodule
