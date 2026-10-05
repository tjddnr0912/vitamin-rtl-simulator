module top;
  parameter signed [7:0] P = -8'sd1;
  if (1) begin : g
    localparam [3:0] P = 4'hF;
    case (-1)
      P: begin : a initial $display("@A outer-misread"); end
      default: begin : d initial $display("@A def"); end
    endcase
    case (P)
      15: begin : b initial $display("@B 15"); end
      -1: begin : c initial $display("@B -1"); end
      default: begin : e initial $display("@B def"); end
    endcase
    case (8'shFF)
      P: begin : f initial $display("@C outer-misread"); end
      default: begin : h initial $display("@C def"); end
    endcase
  end
endmodule
