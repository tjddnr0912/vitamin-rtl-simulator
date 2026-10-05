package pk; localparam [64:0] i = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pk::*;
  for (genvar i = 0; i < 2; i++) begin : g
    localparam int K = i;
    localparam [64:0] KW = i;
    wire [64:0] wv = i;
    initial begin
      #1 $display("val %m i=%0d K=%0d KW=%0d wv=%0d b=%0d", i, K, KW, wv, $bits(i));
      case (i) 0: $display("pc %m zero"); 1: $display("pc %m one"); default: $display("pc %m def"); endcase
    end
  end
  initial #5 $display("post %0d", i);
  initial #100 $finish;
endmodule
