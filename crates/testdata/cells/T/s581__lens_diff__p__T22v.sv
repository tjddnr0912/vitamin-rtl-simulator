module top;
  parameter signed Q4 = 4'hF;
  parameter signed [3:0] Q4r = 4'hF;
  localparam signed Q4l = 4'hF;
  initial begin
    $display("@Q4 %0d bits=%0d lt0=%0d", Q4, $bits(Q4), Q4 < 0);
    $display("@Q4r %0d bits=%0d lt0=%0d", Q4r, $bits(Q4r), Q4r < 0);
    $display("@Q4l %0d bits=%0d lt0=%0d", Q4l, $bits(Q4l), Q4l < 0);
    case (-1) Q4: $display("@pQ4 hit"); default: $display("@pQ4 def"); endcase
  end
  if (Q4 < 0) begin : gi initial $display("@giQ4 neg"); end else begin : ge initial $display("@giQ4 nonneg"); end
  case (-1) Q4l: begin : a initial $display("@gcQ4l hit"); end default: begin : d initial $display("@gcQ4l def"); end endcase
  case (-1) Q4r: begin : a2 initial $display("@gcQ4r hit"); end default: begin : d2 initial $display("@gcQ4r def"); end endcase
endmodule
