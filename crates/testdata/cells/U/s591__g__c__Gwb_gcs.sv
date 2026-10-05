module top;
  if (1) begin : b
    localparam [64:0] i = 65'h1_0000_0000_0000_0009;
    for (genvar i = 0; i < 2; i++) begin : g
      case (i)
        0: begin : z initial #1 $display("gcs %m zero"); end
        1: begin : o initial #1 $display("gcs %m one"); end
        default: begin : d initial #1 $display("gcs %m def"); end
      endcase
    end
  end
  initial #100 $finish;
endmodule
