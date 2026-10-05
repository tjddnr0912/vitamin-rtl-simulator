module top;
  localparam P = 3;
  if (1) begin : b
    localparam [64:0] P = 65'h1_0000_0000_0000_0009;
    case (P)
      65'h1_0000_0000_0000_0009: begin : hw initial #1 $display("gcs wide"); end
      3: begin : h3 initial #1 $display("gcs three"); end
      default: begin : d initial #1 $display("gcs def"); end
    endcase
  end
  initial #100 $finish;
endmodule
