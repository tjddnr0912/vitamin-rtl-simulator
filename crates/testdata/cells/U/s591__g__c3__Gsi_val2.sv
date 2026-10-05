package pk; localparam string i = "AB"; endpackage
module top;
  import pk::*;
  for (genvar i = 0; i < 2; i++) begin : g
    localparam int K = i;
    wire [64:0] wv = i;
    initial #1 $display("val %m i=%0d K=%0d wv=%0d", i, K, wv);
    case (i) 0: begin : z initial #1 $display("gcs %m zero"); end 1: begin : o initial #1 $display("gcs %m one"); end default: begin : d initial #1 $display("gcs %m def"); end endcase
  end
  initial #100 $finish;
endmodule
