module top;
  logic [7:0] mem [0:3];
  wire [7:0] y;
  function logic [7:0] f();
    $display("f t=%0t m0=%h", $time, mem[0]);
    return mem[0];
  endfunction
  assign y = f();
  initial begin
    mem[0] = 8'h5a;
    $display("i0 y=%h", y);
    #1 $display("i1 y=%h", y);
    $finish;
  end
  initial #100 $finish;
endmodule
