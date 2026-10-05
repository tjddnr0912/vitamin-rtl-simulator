module top;
  logic g; logic [1:0] y;
  function logic [1:0] f();
    $display("f t=%0t g=%b", $time, g);
    return {g, ~g};
  endfunction
  assign y = f();
  initial begin
    g = 1;
    #1 $display("t=%0t y=%b", $time, y);
    g = 0;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
