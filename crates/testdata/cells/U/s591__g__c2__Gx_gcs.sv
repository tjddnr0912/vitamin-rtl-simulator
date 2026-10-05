module top;
  for (genvar i = 0; i < 2; i++) begin : g
    case (i)
      0: begin : z initial #1 $display("gcs %m zero"); end
      1: begin : o initial #1 $display("gcs %m one"); end
      default: begin : d initial #1 $display("gcs %m def"); end
    endcase
  end
  initial #100 $finish;
endmodule
