module top;
  logic [1:0] mem [0:3];
  wire [1:0] y;
  function logic [1:0] f();
    unique case (mem[0])
      2'b01: f = 2'b10;
      2'b10: f = 2'b01;
    endcase
  endfunction
  assign y = f();
  initial begin
    mem[0] = 2'b01;
    #1 $display("i1 y=%b", y);
    $finish;
  end
  initial #100 $finish;
endmodule
