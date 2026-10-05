module top;
  for (genvar i = 0; i < 3; i++) begin : g
    localparam [64:0] LW = {63'd0, 2'(i)};
    localparam [64:0] LX = {1'b1, 62'd0, 2'(i)};
    case (i)
      LX: begin : x initial $display("@%0d LX-wrong", i); end
      LW: begin : a initial $display("@%0d a", i); end
      default: begin : d initial $display("@%0d d", i); end
    endcase
  end
endmodule
