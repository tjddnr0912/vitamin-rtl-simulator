module top;
  localparam i = 9;
  for (genvar i = 0; i < 2; i++) begin : g
    case (i)
      0: begin : z initial #1 $display("gcs %m zero"); end
      1: begin : o initial #1 $display("gcs %m one"); end
      default: begin : d initial #1 $display("gcs %m def"); end
    endcase
  end
  initial #5 $display("post %0d", i);
  initial #100 $finish;
endmodule
