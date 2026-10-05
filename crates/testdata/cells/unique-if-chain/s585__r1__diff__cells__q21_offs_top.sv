module top;
  logic a = 0, b = 0, c = 0;
  mb u(.a(a), .b(b), .c(c));
  initial begin
    #1 unique if (a) $display("A a"); else if (b) $display("A b"); else if (c) $display("A c");
    #10 $finish;
  end
endmodule
