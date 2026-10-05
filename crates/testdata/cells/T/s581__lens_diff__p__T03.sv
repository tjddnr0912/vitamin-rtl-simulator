module top;
  parameter [1:0] i = 2'd3;
  for (genvar i = 0; i < 6; i++) begin : g
    case (i)
      4: begin : a initial $display("@%0d a", i); end
      3: begin : b initial $display("@%0d b", i); end
      default: begin : d initial $display("@%0d d", i); end
    endcase
    case (4)
      i: begin : la initial $display("@L%0d a", i); end
      default: begin : ld initial $display("@L%0d d", i); end
    endcase
  end
endmodule
