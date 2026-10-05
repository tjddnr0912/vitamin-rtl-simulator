module top;
  logic clk = 0, resetn = 0;
  logic [31:0] instr;
  localparam bit DEBUG = 1;
  function automatic logic [6:0] dec(input logic [31:0] ins);
    if (DEBUG && ins[1:0] != 2'b11) $display("illegal %h t=%0t", ins, $time);
    dec = ins[6:0];
  endfunction
  wire [6:0] opcode = dec(instr);
  logic [7:0] cnt;
  always @(posedge clk) if (!resetn) cnt <= 0; else cnt <= cnt + (opcode == 7'h13);
  always #5 clk = ~clk;
  initial begin instr = 32'h00000013; #12 resetn = 1; #10 instr = 32'h00000012; #10 instr = 32'h13; end
  initial #50 begin $display("cnt=%0d", cnt); $finish; end
endmodule
