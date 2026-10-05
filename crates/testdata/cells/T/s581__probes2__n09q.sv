module top;
  for (genvar i = 0; i < 1; i++) begin : g
    localparam QQ = i - 1;
    localparam int QI = i - 1;
    initial $display("i=%0d eq=%b bits=%0d qq=%h qqeq=%b qi=%h bitsqq=%0d", i, (i - 1) === 32'hFFFFFFFF, $bits(i - 1), QQ, QQ === 32'hFFFFFFFF, QI, $bits(QQ));
    case (QI)
      32'hFFFFFFFF: begin : c0 initial $display("QI a"); end
      default: begin : cd initial $display("QI def"); end
    endcase
    case (i + 32'sd0 - 32'sd1)
      32'hFFFFFFFF: begin : c1 initial $display("SUM a"); end
      default: begin : cd1 initial $display("SUM def"); end
    endcase
  end
  localparam P0 = 0;
  case (P0 - 1)
    32'hFFFFFFFF: begin : c2 initial $display("P0 a"); end
    default: begin : cd2 initial $display("P0 def"); end
  endcase
  localparam int PI0 = 0;
  case (PI0 - 1)
    32'hFFFFFFFF: begin : c3 initial $display("PI0 a"); end
    default: begin : cd3 initial $display("PI0 def"); end
  endcase
endmodule
