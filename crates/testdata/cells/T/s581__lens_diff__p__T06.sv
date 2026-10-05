module top;
  for (genvar i = 0; i < 3; i++) begin : gi
    for (genvar j = 0; j < 3; j++) begin : gj
      case (i * 3 + j - 4)
        4'sb1111: begin : a initial $display("@%0d%0d a", i, j); end
        32'hFFFF_FFFF: begin : b initial $display("@%0d%0d b", i, j); end
        4'b1100: begin : c initial $display("@%0d%0d c", i, j); end
        -4: begin : e initial $display("@%0d%0d e", i, j); end
        3'sb111 - 3'sb001, 5'd30 + 5'd2: begin : f initial $display("@%0d%0d f", i, j); end
        default: begin : d initial $display("@%0d%0d d", i, j); end
      endcase
    end
  end
endmodule
