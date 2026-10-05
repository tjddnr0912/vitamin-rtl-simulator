module top;
  initial begin
    for (int v = -4; v <= 4; v++)
      case (v)
        4'sb1111: $display("@%0d a", v);
        32'hFFFF_FFFF: $display("@%0d b", v);
        4'b1100: $display("@%0d c", v);
        -4: $display("@%0d e", v);
        3'sb111 - 3'sb001, 5'd30 + 5'd2: $display("@%0d f", v);
        default: $display("@%0d d", v);
      endcase
  end
endmodule
