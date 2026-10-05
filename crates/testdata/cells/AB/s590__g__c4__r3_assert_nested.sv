module top;
  logic a, b; logic [1:0] y;
  function logic [1:0] h(input logic [1:0] v);
    assert (v != 2'b00) else $error("bad v=%b", v);
    return v;
  endfunction
  function logic [1:0] f(input logic x, input logic z);
    return h({x, z});
  endfunction
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
