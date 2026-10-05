module mb(input logic a, input logic b, input logic c);
  initial begin
    #3 unique if (a) $display("B a"); else               assert (b) else              if (c) $display("B c");
  end
endmodule
