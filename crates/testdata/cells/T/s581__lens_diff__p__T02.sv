module top #(parameter int W = 4, parameter logic [W-1:0] P = '1, parameter [64:0] WP = 65'h1);
  case (P)
    8'hFF: begin : a initial $display("@P a"); end
    4'hF:  begin : b initial $display("@P b"); end
    default: begin : d initial $display("@P def"); end
  endcase
  case (1)
    WP: begin : wa initial $display("@WP hit"); end
    default: begin : wd initial $display("@WP def"); end
  endcase
endmodule
