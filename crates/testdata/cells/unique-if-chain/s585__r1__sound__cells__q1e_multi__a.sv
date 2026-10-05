module sub(input logic a, input logic b);
  initial begin
    #1 unique if (a) $display("sa"); else if (b) $display("sb");
    $display("t=%0t sub", $time);
  end
endmodule
